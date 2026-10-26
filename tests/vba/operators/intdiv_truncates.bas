' INV: op.intdiv
' SOURCE: spec ms-vbal 5.6.9.3.6 integer division operator
' EXPECT: -3
' EXPECT: 4
Public Sub Main()
    Debug.Print CStr(-7 \ 2)
    Debug.Print CStr(7.6 \ 2)
End Sub
