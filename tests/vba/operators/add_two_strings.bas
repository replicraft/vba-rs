' INV: op.add, op.concat
' SOURCE: derived + concatenates when both operands are strings (plus-operator page)
' EXPECT: 34
' EXPECT: abcd
Public Sub Main()
    Debug.Print CStr("3" + "4")
    Debug.Print CStr("ab" + "cd")
End Sub
