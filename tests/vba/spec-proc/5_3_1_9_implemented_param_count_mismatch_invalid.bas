' INV: vbal.5.3.1.9, stmt.implements
' SOURCE: spec ms-vbal 5.3.1.9 implemented method has the same number of parameters as the interface member
' ERROR: load
'#MODULE IThing class
Public Sub Work(ByVal n As Long)
End Sub
'#MODULE Impl class
Implements IThing
Private Sub IThing_Work()
End Sub
'#MODULE Module1 standard
Public Sub Main()
End Sub
