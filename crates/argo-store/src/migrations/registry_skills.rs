//! Ordered migration registrations for versions 21..=40.

use super::{
    v21_skill_ab_tests, v22, v23, v24, v25, v26, v27, v28, v29, v30, v31, v32, v33, v34, v35, v36,
    v37, v38, v39, v40, Migration,
};

pub(crate) fn migrations() -> Vec<Migration> {
    vec![
        Migration {
            version: 21,
            up: |tx| Box::pin(v21_skill_ab_tests::up(tx)),
            down: |tx| Box::pin(v21_skill_ab_tests::down(tx)),
        },
        Migration {
            version: 22,
            up: |tx| Box::pin(v22::up(tx)),
            down: |tx| Box::pin(v22::down(tx)),
        },
        Migration {
            version: 23,
            up: |tx| Box::pin(v23::up(tx)),
            down: |tx| Box::pin(v23::down(tx)),
        },
        Migration {
            version: 24,
            up: |tx| Box::pin(v24::up(tx)),
            down: |tx| Box::pin(v24::down(tx)),
        },
        Migration {
            version: 25,
            up: |tx| Box::pin(v25::up(tx)),
            down: |tx| Box::pin(v25::down(tx)),
        },
        Migration {
            version: 26,
            up: |tx| Box::pin(v26::up(tx)),
            down: |tx| Box::pin(v26::down(tx)),
        },
        Migration {
            version: 27,
            up: |tx| Box::pin(v27::up(tx)),
            down: |tx| Box::pin(v27::down(tx)),
        },
        Migration {
            version: 28,
            up: |tx| Box::pin(v28::up(tx)),
            down: |tx| Box::pin(v28::down(tx)),
        },
        Migration {
            version: 29,
            up: |tx| Box::pin(v29::up(tx)),
            down: |tx| Box::pin(v29::down(tx)),
        },
        Migration {
            version: 30,
            up: |tx| Box::pin(v30::up(tx)),
            down: |tx| Box::pin(v30::down(tx)),
        },
        Migration {
            version: 31,
            up: |tx| Box::pin(v31::up(tx)),
            down: |tx| Box::pin(v31::down(tx)),
        },
        Migration {
            version: 32,
            up: |tx| Box::pin(v32::up(tx)),
            down: |tx| Box::pin(v32::down(tx)),
        },
        Migration {
            version: 33,
            up: |tx| Box::pin(v33::up(tx)),
            down: |tx| Box::pin(v33::down(tx)),
        },
        Migration {
            version: 34,
            up: |tx| Box::pin(v34::up(tx)),
            down: |tx| Box::pin(v34::down(tx)),
        },
        Migration {
            version: 35,
            up: |tx| Box::pin(v35::up(tx)),
            down: |tx| Box::pin(v35::down(tx)),
        },
        Migration {
            version: 36,
            up: |tx| Box::pin(v36::up(tx)),
            down: |tx| Box::pin(v36::down(tx)),
        },
        Migration {
            version: 37,
            up: |tx| Box::pin(v37::up(tx)),
            down: |tx| Box::pin(v37::down(tx)),
        },
        Migration {
            version: 38,
            up: |tx| Box::pin(v38::up(tx)),
            down: |tx| Box::pin(v38::down(tx)),
        },
        Migration {
            version: 39,
            up: |tx| Box::pin(v39::up(tx)),
            down: |tx| Box::pin(v39::down(tx)),
        },
        Migration {
            version: 40,
            up: |tx| Box::pin(v40::up(tx)),
            down: |tx| Box::pin(v40::down(tx)),
        },
    ]
}
