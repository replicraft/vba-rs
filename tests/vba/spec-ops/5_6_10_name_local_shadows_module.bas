' INV: vbal.5.6.10, decl.scope
' SOURCE: spec ms-vbal 5.6.10 simple name tiers: procedure namespace precedes enclosing module namespace
' EXPECT: local
' EXPECT: module
Private x As String
Private Sub SetIt()
    x = "module"
End Sub
Private Sub Show()
    Debug.Print x
End Sub
Public Sub Main()
    Dim x As String
    x = "local"
    SetIt
    Debug.Print x
    Show
End Sub
