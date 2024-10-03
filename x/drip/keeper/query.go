package keeper

import (
	"protochainmuffin/x/drip/types"
)

var _ types.QueryServer = Keeper{}
