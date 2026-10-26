' INV: op.intdiv
' SOURCE: spec ms-vbal 5.6.9.3.6 integer division operator
' EXPECT: 2
' EXPECT: 3
' EXPECT: 33
Public Sub Main()
    Debug.Print CStr(11 \ 4)
    Debug.Print CStr(9 \ 3)
    Debug.Print CStr(100 \ 3)
End Sub
