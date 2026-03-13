INSERT INTO cn_search.runtime_flags (flag_name, flag_value, updated_by)
VALUES
    (
        'suggest_rerank_mode',
        'shadow',
        'migration'
    ),
    (
        'suggest_relation_weights',
        '{"is_member":1.20,"is_following_community":0.80,"friends_member_count":0.35,"two_hop_follow_count":0.25,"last_view_decay":0.15,"muted_or_blocked":-1.00}',
        'migration'
    )
ON CONFLICT (flag_name) DO NOTHING;
