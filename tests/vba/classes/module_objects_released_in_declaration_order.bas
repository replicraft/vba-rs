' INV: decl.class-events, decl.scope
' SOURCE: derived shutdown releases module-level objects; the order is the declaration order (documented in the runtime notes, behavior on Office not verified)
' EXPECT: term a
' EXPECT: term b
'#MODULE Res class
Public Name As String
Private Sub Class_Terminate()
    Debug.Print "term " & Name
End Sub
'#MODULE Module1 standard
Dim a As Res, b As Res
Public Sub Main()
    Set a = New Res: a.Name = "a"
    Set b = New Res: b.Name = "b"
End Sub
