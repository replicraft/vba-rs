' INV: vbal.5.2.3.2
' SOURCE: spec ms-vbal 5.2.3.2 constants in a module have distinct names
' ERROR: load
Private Const a As Long = 1
Public Const a As Long = 2
Public Sub Main()
End Sub
