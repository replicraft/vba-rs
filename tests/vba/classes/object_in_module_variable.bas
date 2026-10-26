' INV: decl.scope, stmt.set
' SOURCE: derived module-level objects live until shutdown; Class_Terminate runs then
' EXPECT: main done
' EXPECT: term
'#MODULE Res class
Private Sub Class_Terminate()
    Debug.Print "term"
End Sub
'#MODULE Module1 standard
Dim keep As Res
Public Sub Main()
    Set keep = New Res
    Debug.Print "main done"
End Sub
