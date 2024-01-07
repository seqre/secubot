// @generated automatically by Diesel CLI.

diesel::table! {
    hall_of_fame_entries (id) {
        id -> Integer,
        hof_id -> Integer,
        user_id -> BigInt,
        description -> Nullable<Text>,
        creation_date -> Text,
    }
}

diesel::table! {
    hall_of_fame_tables (id) {
        id -> Integer,
        guild_id -> BigInt,
        title -> Text,
        description -> Nullable<Text>,
        creation_date -> Text,
    }
}

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

diesel::joinable!(hall_of_fame_entries -> hall_of_fame_tables (hof_id));

diesel::allow_tables_to_appear_in_same_query!(
    hall_of_fame_entries,
    hall_of_fame_tables,
    todos,
);
