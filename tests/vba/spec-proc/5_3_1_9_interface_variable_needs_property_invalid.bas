' INV: vbal.5.3.1.9, stmt.implements
' SOURCE: spec ms-vbal 5.3.1.9 a public variable of the interface must be implemented by a property
' ERROR: load
'#MODULE IThing class
Public Title As String
'#MODULE Impl class
Implements IThing
Private Sub IThing_Title()
End Sub
'#MODULE Module1 standard
Public Sub Main()
End Sub
