import {Button, Chip, Container, Group, Table} from '@mantine/core';
import {useLocalStorage} from '@mantine/hooks';
import {useState} from 'react';
import {BudgetListItem} from '../rest-model';
import {Heading} from '../components/basic/Heading';
import {useTranslation} from 'react-i18next';
import useSWR from 'swr';
import {fetcher} from '../index';
import {groupBy, sortBy} from 'lodash';
import BudgetCategory from '../components/budget/BudgetCategory';

export default function Budgets() {
    const {t} = useTranslation();
    const [hideZeroAssignBudget, setHideZeroAssignBudget] = useLocalStorage({
        key: 'hideZeroAssignBudget',
        defaultValue: false,
    });

    const [date, setDate] = useState<Date>(new Date());

    const {
        data: budgets,
        error
    } = useSWR<BudgetListItem[]>(`/api/budgets?year=${date.getFullYear()}&month=${date.getMonth() + 1}`, fetcher);
    if (error) return <div>failed to load</div>;
    if (!budgets) return <div>loading...</div>;

    return (
        <Container fluid>
            <Heading title={`Budgets`}></Heading>
            <Group my="lg">
                <Button variant="outline" color="gray" radius="xl" size="xs">
                    {t('REFRESH')}
                </Button>
                <Chip checked={hideZeroAssignBudget} onChange={() => setHideZeroAssignBudget(!hideZeroAssignBudget)}>
                    Hide Zero Amount Assigned Budget
                </Chip>
            </Group>
            <Table verticalSpacing="xs" withBorder>
                <thead>
                <tr>
                    <th>Category</th>
                    <th style={{textAlign: 'end'}}>Assigned</th>
                    <th style={{textAlign: 'end'}}>Activity</th>
                    <th style={{textAlign: 'end'}}>Available</th>
                </tr>
                </thead>
                <tbody>
                {sortBy(Object.entries(groupBy(budgets, (budget) => budget.category)), (entry) => entry[0]).map((entry) => (
                    <BudgetCategory name={entry[0]} items={entry[1]}></BudgetCategory>
                ))}
                </tbody>
            </Table>
        </Container>
    );
}
