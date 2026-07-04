package docker

import rego.v1

local_allowlist := {"dockerfile", "context"}

default allow := false

allow if {
	input.image.isCanonical
	input.image.checksum
}

allow if {
	input.local.name in local_allowlist
}

allow if {
	input.git.remote == "https://github.com/pndrik/beaver.git"
}

deny_msg contains msg if {
	input.image
	not allow
	msg := sprintf("image '%s:%s' is not pinned to a digest", [input.image.repo, input.image.tag])
}

deny_msg contains msg if {
	input.local
	not allow
	msg := sprintf("adding files from '%s' is not allowed", [input.local.name])
}

deny_msg contains msg if {
	input.http
	msg := sprintf("fetching from '%s' is not allowed", [input.http.host])
}

deny_msg contains msg if {
	input.git
	not allow
	msg := sprintf("fetching from '%s' is not allowed", [input.git.remote])
}

deny_msg contains msg if {
	not allow
	not input.image
	not input.local
	not input.http
	msg := sprintf("input type not covered by policy: %v", [input])
}

decision := {"allow": allow, "deny_msg": deny_msg}
