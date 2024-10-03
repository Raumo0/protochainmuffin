package types

const (
	// ModuleName defines the module name
	ModuleName = "drip"

	// StoreKey defines the primary module store key
	StoreKey = ModuleName

	// MemStoreKey defines the in-memory store key
	MemStoreKey = "mem_drip"
)

var (
	ParamsKey = []byte("p_drip")
)

func KeyPrefix(p string) []byte {
	return []byte(p)
}
