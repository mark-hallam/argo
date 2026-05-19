//! Ordered migration registrations for versions 57..=74.

use super::{
    v57, v58, v59, v60, v61, v62, v63, v64, v65, v66, v67, v68, v69, v70, v71, v72, v73, v74,
    Migration,
};

pub(crate) fn migrations() -> Vec<Migration> {
    vec![
        Migration {
            version: 57,
            up: |tx| Box::pin(v57::up(tx)),
            down: |tx| Box::pin(v57::down(tx)),
        },
        Migration {
            version: 58,
            up: |tx| Box::pin(v58::up(tx)),
            down: |tx| Box::pin(v58::down(tx)),
        },
        Migration {
            version: 59,
            up: |tx| Box::pin(v59::up(tx)),
            down: |tx| Box::pin(v59::down(tx)),
        },
        Migration {
            version: 60,
            up: |tx| Box::pin(v60::up(tx)),
            down: |tx| Box::pin(v60::down(tx)),
        },
        Migration {
            version: 61,
            up: |tx| Box::pin(v61::up(tx)),
            down: |tx| Box::pin(v61::down(tx)),
        },
        Migration {
            version: 62,
            up: |tx| Box::pin(v62::up(tx)),
            down: |tx| Box::pin(v62::down(tx)),
        },
        Migration {
            version: 63,
            up: |tx| Box::pin(v63::up(tx)),
            down: |tx| Box::pin(v63::down(tx)),
        },
        Migration {
            version: 64,
            up: |tx| Box::pin(v64::up(tx)),
            down: |tx| Box::pin(v64::down(tx)),
        },
        Migration {
            version: 65,
            up: |tx| Box::pin(v65::up(tx)),
            down: |tx| Box::pin(v65::down(tx)),
        },
        Migration {
            version: 66,
            up: |tx| Box::pin(v66::up(tx)),
            down: |tx| Box::pin(v66::down(tx)),
        },
        Migration {
            version: 67,
            up: |tx| Box::pin(v67::up(tx)),
            down: |tx| Box::pin(v67::down(tx)),
        },
        Migration {
            version: 68,
            up: |tx| Box::pin(v68::up(tx)),
            down: |tx| Box::pin(v68::down(tx)),
        },
        Migration {
            version: 69,
            up: |tx| Box::pin(v69::up(tx)),
            down: |tx| Box::pin(v69::down(tx)),
        },
        Migration {
            version: 70,
            up: |tx| Box::pin(v70::up(tx)),
            down: |tx| Box::pin(v70::down(tx)),
        },
        Migration {
            version: 71,
            up: |tx| Box::pin(v71::up(tx)),
            down: |tx| Box::pin(v71::down(tx)),
        },
        Migration {
            version: 72,
            up: |tx| Box::pin(v72::up(tx)),
            down: |tx| Box::pin(v72::down(tx)),
        },
        Migration {
            version: 73,
            up: |tx| Box::pin(v73::up(tx)),
            down: |tx| Box::pin(v73::down(tx)),
        },
        Migration {
            version: 74,
            up: |tx| Box::pin(v74::up(tx)),
            down: |tx| Box::pin(v74::down(tx)),
        },
    ]
}
