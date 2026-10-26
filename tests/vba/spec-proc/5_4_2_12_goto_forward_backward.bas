' INV: vbal.5.4.2.12, stmt.goto
' SOURCE: spec ms-vbal 5.4.2.12 GoTo branches to the statement after the label (forward and backward; Go To spelling)
' EXPECT: n=3
' EXPECT: end
Public Sub Main()
    Dim n As Long
Again:
    n = n + 1
    If n < 3 Then GoTo Again
    GoTo Fin
    Debug.Print "skipped"
Fin:
    Debug.Print "n=" & CStr(n)
    Debug.Print "end"
End Sub
