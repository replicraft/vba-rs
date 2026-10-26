' INV: vbal.5.2.1.1
' SOURCE: spec ms-vbal 5.2.1.1 Option Compare Binary
' EXPECT: eq=False
' EXPECT: lt=False
Option Compare Binary
Public Sub Main()
    Debug.Print "eq=" & CStr("abc" = "ABC")
    Debug.Print "lt=" & CStr("a" < "B")
End Sub
