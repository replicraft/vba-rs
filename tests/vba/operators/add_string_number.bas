' INV: op.add
' SOURCE: derived + adds when one operand is numeric and the other a numeric string (plus-operator page)
' EXPECT: 7
' EXPECT: 7
Public Sub Main()
    Debug.Print CStr("3" + 4)
    Debug.Print CStr(4 + "3")
End Sub
