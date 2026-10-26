' INV: func.mid
' SOURCE: spec ms-vbal 6.1.2.11.1.25 Mid start and length
' EXPECT: Mid
' EXPECT: Demo
' EXPECT: Function Demo
Public Sub Main()
    Debug.Print CStr(Mid("Mid Function Demo", 1, 3))
    Debug.Print CStr(Mid("Mid Function Demo", 14, 4))
    Debug.Print CStr(Mid("Mid Function Demo", 5))
End Sub
