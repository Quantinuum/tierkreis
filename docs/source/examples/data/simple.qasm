OPENQASM 2.0;
include "hqslib1.inc";

qreg q[2];
creg c[2];
U1q(0.20000000000000012*pi,0.0*pi) q[0];
U1q(0.5*pi,0.5*pi) q[1];
RZZ(0.5*pi) q[0],q[1];
measure q[0] -> c[0];
U1q(0.5*pi,0.0*pi) q[1];
measure q[1] -> c[1];