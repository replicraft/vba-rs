' INV: op.not
' SOURCE: derived Not is bitwise on integers: Not x = -x - 1
' EXPECT: -6
' EXPECT: -1
' EXPECT: 0
Public Sub Main()
    Debug.Print CStr(Not 5)
    Debug.Print CStr(Not 0)
    Debug.Print CStr(Not -1)
End Sub
