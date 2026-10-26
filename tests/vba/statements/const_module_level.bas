' INV: stmt.const, decl.scope
' SOURCE: spec ms-vbal 5.2.3.2 module Const declarations
' EXPECT:  100 x
Private Const LIMIT As Long = 100
Public Const NAME_ = "x"
Public Sub Main()
    Debug.Print LIMIT; NAME_
End Sub
