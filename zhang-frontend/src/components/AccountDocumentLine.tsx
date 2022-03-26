import { Badge, Flex, Text } from "@chakra-ui/react";
import { useNavigate } from "react-router";

interface Props {
    filename: string
    account: {
        name: string,
        status: string
    }
}

export default function Component({ filename, account }: Props) {
    let navigate = useNavigate();
    const extension = filename.split(".").pop()?.toUpperCase();

    return (
        <Flex m={2} justifyContent="space-between">
            <Flex alignItems={"center"} alignContent="center">
                <Badge variant='outline'>{extension}</Badge>
                <Text mx={2}>{filename}</Text>
            </Flex>
            <Text mx={1} cursor="pointer" onClick={() => navigate(`/accounts/${account.name}`)}>{account.name}</Text>
        </Flex>
    )
}