' INV: stmt.gosub-return
' SOURCE: spec ms-vbal 5.4.2.14 GoSub statement, 5.4.2.15 Return statement
' EXPECT: hi
' EXPECT: back
Public Sub Main()
    GoSub Sayhi
    Debug.Print "back"
    Exit Sub
Sayhi:
    Debug.Print "hi"
    Return
End Sub
