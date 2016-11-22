# snapshot_manager
A helper for creating snapshots on ZFS on Linux on behalf unprivileged users over the network.

**Warning**: Don't use this project just yet. It is still work in progress and will change
drastically over the following 0.\* releases. Integrating further snapshot management and potentially
btrfs support might follow including backward incompatible changes.

## Architecture
A HTTP server listens to requests to create snapshots.
A whitelist specifies all allowed volumes to back up.
Snapshots are created with the day's date and a call to the zfs binary.
As with ZFS on Linux this is only possible as root the server has to run as root.

`curl "localhost:7877?zroot/my/subvolume"` will request `zroot/my/subvolume` to be snapshotted.
Note that currently only one snapshot per day can be created.
(I will think about ways how this can be improved in a safe manner, i.e. no snapshot spam.)

