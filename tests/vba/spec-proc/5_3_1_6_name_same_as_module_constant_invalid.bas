' INV: vbal.5.3.1.6
' SOURCE: spec ms-vbal 5.3.1.6 procedure name may not equal a module constant name
' ERROR: load
Const Limit As Long = 5
Function Limit() As Long
End Function
Public Sub Main()
End Sub
