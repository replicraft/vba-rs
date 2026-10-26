' INV: op.precedence, op.not, op.and, op.or
' SOURCE: spec ms-vbal 5.6.9.1 operator precedence
' EXPECT: True
' EXPECT: True
' EXPECT: True
' EXPECT: True
Public Sub Main()
    Debug.Print CStr(Not True Or True)
    Debug.Print CStr(True Or False And False)
    Debug.Print CStr(Not 1 = 2)
    Debug.Print CStr(1 + 2 = 3 And 2 < 3)
End Sub
