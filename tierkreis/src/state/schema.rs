// @generated automatically by Diesel CLI.

diesel::table! {
    node_outputs (id) {
        id -> Text,
        node_state_id -> Text,
        name -> Text,
        asset_location -> Text,
    }
}

diesel::table! {
    node_states (id) {
        id -> Text,
        run_id -> Text,
        attempt -> Integer,
        node_location -> Text,
        started_time -> Timestamp,
        finished_time -> Nullable<Timestamp>,
        error -> Nullable<Text>,
        error_detail -> Nullable<Text>,
    }
}

diesel::table! {
    workflow_runs (id, attempt) {
        id -> Text,
        attempt -> Integer,
        workflow_id -> Text,
        run_metadata -> Text,
        status -> Text,
        started_at -> Timestamp,
    }
}

diesel::table! {
    workflows (id) {
        id -> Text,
        name -> Nullable<Text>,
        created_at -> Timestamp,
    }
}

diesel::joinable!(node_outputs -> node_states (node_state_id));
diesel::joinable!(workflow_runs -> workflows (workflow_id));

diesel::allow_tables_to_appear_in_same_query!(node_outputs, node_states, workflow_runs, workflows,);
