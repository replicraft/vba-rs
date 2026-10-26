' INV: func.mid
' SOURCE: spec ms-vbal 6.1.2.11.1.25 Mid length fewer than requested, start beyond end
' EXPECT: 
' EXPECT: bc
' EXPECT: c
' EXPECT: 
Public Sub Main()
    Debug.Print CStr(Mid("abc", 2, 0))
    Debug.Print CStr(Mid("abc", 2, 100))
    Debug.Print CStr(Mid("abc", 3, 1))
    Debug.Print CStr(Mid("", 1))
End Sub
