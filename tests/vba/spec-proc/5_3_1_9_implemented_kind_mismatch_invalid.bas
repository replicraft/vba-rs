' INV: vbal.5.3.1.9, stmt.implements
' SOURCE: spec ms-vbal 5.3.1.9 implemented method must be the same kind as the interface member
' ERROR: load
'#MODULE IThing class
Public Function Work() As Long
End Function
'#MODULE Impl class
Implements IThing
Private Sub IThing_Work()
End Sub
'#MODULE Module1 standard
Public Sub Main()
End Sub
