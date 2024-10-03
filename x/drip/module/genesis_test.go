package drip_test

import (
	"testing"

	keepertest "protochainmuffin/testutil/keeper"
	"protochainmuffin/testutil/nullify"
	drip "protochainmuffin/x/drip/module"
	"protochainmuffin/x/drip/types"

	"github.com/stretchr/testify/require"
)

func TestGenesis(t *testing.T) {
	genesisState := types.GenesisState{
		Params: types.DefaultParams(),

		// this line is used by starport scaffolding # genesis/test/state
	}

	k, ctx := keepertest.DripKeeper(t)
	drip.InitGenesis(ctx, k, genesisState)
	got := drip.ExportGenesis(ctx, k)
	require.NotNil(t, got)

	nullify.Fill(&genesisState)
	nullify.Fill(got)

	// this line is used by starport scaffolding # genesis/test/assert
}
