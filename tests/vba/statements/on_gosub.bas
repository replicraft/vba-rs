' INV: stmt.on-gosub-on-goto, stmt.gosub-return
' SOURCE: spec ms-vbal 5.4.2.16 On GoSub statement
' EXPECT: b
Public Sub Main()
    On 2 GoSub A, B
    Exit Sub
A:
    Debug.Print "a"
    Return
B:
    Debug.Print "b"
    Return
End Sub
