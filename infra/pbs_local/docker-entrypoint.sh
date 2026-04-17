#!/bin/bash
set -e

# Source basic PBS environment
export PBS_CONF_FILE=/etc/pbs.conf
export PBS_EXEC=/opt/pbs
export PBS_HOME=/var/spool/pbs
export PATH=/opt/pbs/bin:/opt/pbs/sbin:$PATH
export LD_LIBRARY_PATH=/opt/pbs/lib:$LD_LIBRARY_PATH


if [ "$1" = "server" ]; then

    mkdir -p /var/spool/pbs/server_priv/security
    chown root:root /var/spool/pbs/server_priv/security
    chmod 700 /var/spool/pbs/server_priv/security

    
    echo "---> Configuring PBS Server ..."
    sed -i 's/PBS_START_SERVER=0/PBS_START_SERVER=1/' /etc/pbs.conf
    sed -i 's/PBS_START_COMM=0/PBS_START_COMM=1/' /etc/pbs.conf
    sed -i 's/PBS_START_SCHED=0/PBS_START_SCHED=1/' /etc/pbs.conf
    source /etc/pbs.conf
    eval $(/opt/pbs/libexec/pbs_db_env)
    /etc/init.d/pbs start
    sleep 10

    echo "---> Setting up Nodes ..."
    qmgr -c "create node p1"
    qmgr -c "create node p2"


    mkdir -p /var/spool/pbs/sched_logs
    touch /var/spool/pbs/sched_logs/$(date +%Y%m%d)
    exec tail -f /var/spool/pbs/sched_logs/$(date +%Y%m%d)
fi

if [ "$1" = "mom" ]; then
    sed -i 's/PBS_START_MOM=0/PBS_START_MOM=1/' /etc/pbs.conf
    
    sleep 10
    until 2>/dev/null >/dev/tcp/server/15001; do sleep 2; done
    source /etc/pbs.conf

    pbs_mom -M 15002 -R 15003
    echo "---> Node $HOSTNAME started."
    mkdir -p /var/spool/pbs/node_logs
    touch /var/spool/pbs/node_logs/$(date +%Y%m%d)
    exec tail -f /var/spool/pbs/node_logs/$(date +%Y%m%d)
fi

exec "$@"
