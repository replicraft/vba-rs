' INV: op.add, op.concat
' SOURCE: spec ms-vbal 5.6.9.3.2 plus operator (String)
' EXPECT: 34
' EXPECT: abcd
Public Sub Main()
    Debug.Print CStr("3" + "4")
    Debug.Print CStr("ab" + "cd")
End Sub
