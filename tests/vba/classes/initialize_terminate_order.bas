' INV: decl.class-events, expr.new, lit.nothing
' SOURCE: derived Class_Initialize runs at first use, Class_Terminate when the last reference goes
' EXPECT: init
' EXPECT: using
' EXPECT: term
' EXPECT: end
'#MODULE Res class
Private Sub Class_Initialize()
    Debug.Print "init"
End Sub
Private Sub Class_Terminate()
    Debug.Print "term"
End Sub
'#MODULE Module1 standard
Public Sub Main()
    Dim r As Res
    Set r = New Res
    Debug.Print "using"
    Set r = Nothing
    Debug.Print "end"
End Sub
