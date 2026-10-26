' INV: op.add
' SOURCE: spec ms-vbal 5.6.9.3.2 plus operator
' EXPECT: 5
' EXPECT: 2.75
' EXPECT: 0
Public Sub Main()
    Debug.Print CStr(2 + 3)
    Debug.Print CStr(2.5 + 0.25)
    Debug.Print CStr(-1 + 1)
End Sub
