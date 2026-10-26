' INV: vbal.5.2.3.2
' SOURCE: spec ms-vbal 5.2.3.2 constant value MUST be Let-coercible to the declared type
' ERROR: load
Private Const a As Long = "abc"
Public Sub Main()
End Sub
