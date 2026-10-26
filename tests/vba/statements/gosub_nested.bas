' INV: stmt.gosub-return
' SOURCE: spec ms-vbal 5.4.2.14 GoSub resumption list, 5.4.2.15 Return statement
' EXPECT: a1
' EXPECT: b
' EXPECT: a2
Public Sub Main()
    GoSub A
    Exit Sub
A:
    Debug.Print "a1"
    GoSub B
    Debug.Print "a2"
    Return
B:
    Debug.Print "b"
    Return
End Sub
