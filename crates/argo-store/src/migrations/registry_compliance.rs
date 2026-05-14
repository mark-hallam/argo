//! Ordered migration registrations for versions 57..=68.

use super::{v57, v58, v59, v60, v61, v62, v63, v64, v65, v66, v67, v68, Migration};

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
    ]
}
