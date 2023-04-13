// @generated automatically by Diesel CLI.

diesel::table! {
    todos (channel_id, id) {
        channel_id -> BigInt,
        id -> Integer,
        todo -> Text,
        creation_date -> Text,
        completion_date -> Nullable<Text>,
        assignee -> Nullable<BigInt>,
        priority -> Integer,
    }
}
