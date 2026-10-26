' INV: decl.class-events
' SOURCE: derived a local object is released when the procedure ends
' EXPECT: use
' EXPECT: term
' EXPECT: after
'#MODULE Res class
Public Sub Touch()
End Sub
Private Sub Class_Terminate()
    Debug.Print "term"
End Sub
'#MODULE Module1 standard
Sub Use()
    Dim r As New Res
    r.Touch
    Debug.Print "use"
End Sub
Public Sub Main()
    Use
    Debug.Print "after"
End Sub
