' INV: stmt.if-then-else
' SOURCE: spec ms-vbal 5.4.2.9 Single-line If statement
' EXPECT: big
' EXPECT: small
Public Sub Main()
    Dim x As Long
    x = 5
    If x > 3 Then Debug.Print "big" Else Debug.Print "small"
    If x > 9 Then Debug.Print "big" Else Debug.Print "small"
End Sub
