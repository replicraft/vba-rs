' INV: decl.class-events, stmt.set
' SOURCE: derived terminate runs only when the last reference is released
' EXPECT: one left
' EXPECT: term
' EXPECT: none
'#MODULE Res class
Private Sub Class_Terminate()
    Debug.Print "term"
End Sub
'#MODULE Module1 standard
Public Sub Main()
    Dim a As Res, b As Res
    Set a = New Res
    Set b = a
    Set a = Nothing
    Debug.Print "one left"
    Set b = Nothing
    Debug.Print "none"
End Sub
