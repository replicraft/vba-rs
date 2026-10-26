' INV: vbal.5.3.1.9, stmt.implements
' SOURCE: spec ms-vbal 5.3.1.9 implemented function has the same declared type as the interface member
' ERROR: load
'#MODULE IThing class
Public Function Work() As Long
End Function
'#MODULE Impl class
Implements IThing
Private Function IThing_Work() As String
End Function
'#MODULE Module1 standard
Public Sub Main()
End Sub
