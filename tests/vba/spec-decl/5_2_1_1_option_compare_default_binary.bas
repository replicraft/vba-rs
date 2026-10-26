' INV: vbal.5.2.1.1
' SOURCE: spec ms-vbal 5.2.1.1 Option Compare default is binary
' EXPECT: eq=False
' EXPECT: lt=False
Public Sub Main()
    Debug.Print "eq=" & CStr("abc" = "ABC")
    Debug.Print "lt=" & CStr("a" < "B")
End Sub
