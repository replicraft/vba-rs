' INV: vbal.5.4.2.12, stmt.goto
' SOURCE: spec ms-vbal 5.4.2.12 GoTo to the end-label completes the procedure body
' EXPECT: before
' EXPECT: caller-continues
Sub S()
    Debug.Print "before"
    GoTo Done
    Debug.Print "never"
Done:
End Sub
Public Sub Main()
    S
    Debug.Print "caller-continues"
End Sub
