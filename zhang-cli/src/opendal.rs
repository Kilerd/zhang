use std::collections::VecDeque;
use std::path::PathBuf;
use std::str::FromStr;

use async_recursion::async_recursion;
use log::{debug, error, info};
use opendal::services::{Fs, Webdav};
use opendal::{ErrorKind, Operator};

use beancount::Beancount;
use zhang_ast::{Directive, Include, Spanned, ZhangString};
use zhang_core::error::IoErrorIntoZhangError;
use zhang_core::exporter::Exporter;
use zhang_core::ledger::Ledger;
use zhang_core::text::exporter::TextExporter;
use zhang_core::text::parser::parse as zhang_parse;
use zhang_core::transform::{TextFileBasedTransformer, TransformResult, Transformer};
use zhang_core::utils::has_path_visited;
use zhang_core::{utils, ZhangError, ZhangResult};

use crate::{DataSource, ServerOpts};

pub struct OpendalTextTransformer {
    operator: Operator,
    data_type: Box<dyn Exporter<Output = String> + 'static + Send + Sync>,
    is_beancount: bool,
}

impl OpendalTextTransformer {
    #[async_recursion]
    async fn append_directive(&self, ledger: &Ledger, directive: Directive, file: Option<PathBuf>, check_file_visit: bool) -> ZhangResult<()> {
        let (entry, main_file_endpoint) = &ledger.entry;

        let endpoint = if let Some(file) = file {
            file
        } else if let Some(datetime) = directive.datetime() {
            let folder = datetime.format("data/%Y/").to_string();

            self.operator.create_dir(&folder).await.expect("cannot create dir");

            let path = format!("data/{}.zhang", datetime.format("%Y/%m"));
            entry.join(PathBuf::from(path))
        } else {
            entry.join(main_file_endpoint)
        };
        let striped_endpoint = endpoint.strip_prefix(entry).expect("cannot strip entry prefix");

        if !has_path_visited(&ledger.visited_files, &endpoint) && check_file_visit {
            let path = match endpoint.strip_prefix(entry) {
                Ok(relative_path) => relative_path.to_str().unwrap(),
                Err(_) => endpoint.to_str().unwrap(),
            };
            self.append_directive(
                ledger,
                Directive::Include(Include {
                    file: ZhangString::QuoteString(path.to_string()),
                }),
                None,
                false,
            )
            .await?;
        }

        let content_buf = ledger.transformer.async_get_content(striped_endpoint.to_string_lossy().to_string()).await?;
        let content = String::from_utf8(content_buf)?;

        let appended_content = format!("{}\n{}\n", content, self.data_type.export_directive(directive));

        ledger
            .transformer
            .async_save_content(ledger, striped_endpoint.to_string_lossy().to_string(), appended_content.as_bytes())
            .await?;
        Ok(())
    }
    pub async fn from_env(source: DataSource, x: &ServerOpts) -> OpendalTextTransformer {
        let operator = match source {
            DataSource::Fs => {
                let mut builder = Fs::default();
                builder.root(x.path.to_string_lossy().to_string().as_str());
                // Operator::new(builder).unwrap().finish()
                Operator::new(builder).unwrap().finish()
            }
            DataSource::WebDav => {
                let mut webdav_builder = Webdav::default();
                webdav_builder.endpoint(&std::env::var("ZHANG_WEBDAV_ENDPOINT").expect("ZHANG_WEBDAV_ENDPOINT must be set"));
                webdav_builder.root(&std::env::var("ZHANG_WEBDAV_ROOT").expect("ZHANG_WEBDAV_ROOT must be set"));
                webdav_builder.username(std::env::var("ZHANG_WEBDAV_USERNAME").ok().as_deref().unwrap_or_default());
                webdav_builder.password(std::env::var("ZHANG_WEBDAV_PASSWORD").ok().as_deref().unwrap_or_default());
                Operator::new(webdav_builder).unwrap().finish()
            }
            _ => {
                todo!()
            }
        };
        let is_beancount = match PathBuf::from(&x.endpoint)
            .extension()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string()
            .as_str()
        {
            "bc" | "bean" => true,
            "zhang" => false,
            _ => unreachable!(),
        };
        let data_type: Box<dyn Exporter<Output = String> + Send + Sync> = if is_beancount { Box::new(Beancount {}) } else { Box::new(TextExporter {}) };

        Self {
            operator,
            data_type,
            is_beancount,
        }
    }

    fn parse(&self, content: &str, path: PathBuf) -> ZhangResult<Vec<Spanned<Directive>>> {
        if self.is_beancount {
            let beancount_parser = beancount::Beancount {};
            beancount_parser
                .parse(content, path)
                .map_err(|it| ZhangError::PestError(it.to_string()))
                .and_then(|data| beancount_parser.transform(data))
        } else {
            zhang_parse(content, path).map_err(|it| ZhangError::PestError(it.to_string()))
        }
    }
    fn go_next(&self, directive: &Spanned<Directive>) -> Option<String> {
        match &directive.data {
            Directive::Include(include) => Some(include.file.clone().to_plain_string()),
            _ => None,
        }
    }
    fn transform(&self, directives: Vec<Spanned<Directive>>) -> ZhangResult<Vec<Spanned<Directive>>> {
        Ok(directives)
    }
    async fn get_file_content(&self, path: PathBuf) -> ZhangResult<String> {
        let path = path.to_str().expect("cannot convert path to string");

        let vec = self.async_get_content(path.to_string()).await.expect("cannot read file");
        Ok(String::from_utf8(vec).expect("invalid utf8 content"))
    }
}

#[async_trait::async_trait]
impl Transformer for OpendalTextTransformer {
    fn load(&self, _entry: PathBuf, _endpoint: String) -> ZhangResult<TransformResult> {
        unimplemented!()
    }

    fn get_content(&self, _path: String) -> ZhangResult<Vec<u8>> {
        unimplemented!()
    }

    fn append_directives(&self, _ledger: &Ledger, _directives: Vec<Directive>) -> ZhangResult<()> {
        unimplemented!()
    }

    fn save_content(&self, _ledger: &Ledger, _path: String, _content: &[u8]) -> ZhangResult<()> {
        unimplemented!()
    }

    async fn async_load(&self, entry: PathBuf, endpoint: String) -> ZhangResult<TransformResult> {
        let entry = entry.canonicalize().with_path(&entry)?;
        let main_endpoint = entry.join(endpoint);
        let main_endpoint = main_endpoint.canonicalize().with_path(&main_endpoint)?;

        let mut load_queue: VecDeque<PathBuf> = VecDeque::new();
        load_queue.push_back(main_endpoint);

        let mut visited: Vec<PathBuf> = Vec::new();
        let mut directives = vec![];
        while let Some(pathbuf) = load_queue.pop_front() {
            let striped_pathbuf = &pathbuf.strip_prefix(&entry).expect("Cannot strip entry").to_path_buf();
            debug!("visited entry file: {:?}", striped_pathbuf.display());

            if utils::has_path_visited(&visited, &pathbuf) {
                continue;
            }
            let file_content = self.get_file_content(striped_pathbuf.clone()).await?;
            let entity_directives = self.parse(&file_content, striped_pathbuf.clone())?;

            entity_directives.iter().filter_map(|directive| self.go_next(directive)).for_each(|buf| {
                let fullpath = if buf.starts_with('/') {
                    PathBuf::from_str(&buf).unwrap()
                } else {
                    pathbuf.parent().map(|it| it.join(buf)).unwrap()
                };
                load_queue.push_back(fullpath);
            });
            directives.extend(entity_directives);
            visited.push(pathbuf);
        }
        Ok(TransformResult {
            directives: self.transform(directives)?,
            visited_files: visited,
        })
    }

    async fn async_get_content(&self, path: String) -> ZhangResult<Vec<u8>> {
        let path_for_read = path.to_owned();
        let result = self.operator.read(&path_for_read).await;
        match result {
            Ok(data) => Ok(data),
            Err(err) => {
                if err.kind() == ErrorKind::NotFound {
                    Ok(Vec::new())
                } else {
                    error!("cannot get content from {}", &path);
                    Ok(Vec::new())
                }
            }
        }
    }

    async fn async_append_directives(&self, ledger: &Ledger, directives: Vec<Directive>) -> ZhangResult<()> {
        for directive in directives {
            self.append_directive(ledger, directive, None, true).await?;
        }
        Ok(())
    }

    async fn async_save_content(&self, _ledger: &Ledger, path: String, content: &[u8]) -> ZhangResult<()> {
        info!("[opendal] save content path={}", &path);
        let vec = content.to_vec();

        self.operator.write(&path, vec).await.expect("cannot write");
        Ok(())
    }
}
