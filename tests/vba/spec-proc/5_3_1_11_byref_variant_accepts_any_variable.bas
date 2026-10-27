' INV: vbal.5.3.1.11, decl.byref-byval
' SOURCE: spec ms-vbal 5.3.1.11 ByRef Variant parameter binds to a variable of any declared type
' EXPECT: Long 6
' EXPECT: String 6
Sub Add(ByRef v As Variant)
    v = v + 1
End Sub
Public Sub Main()
    Dim n As Long, s As String
    n = 5
    s = "5"
    Add n
    Add s
    Debug.Print TypeName(n) & " " & CStr(n)
    Debug.Print TypeName(s) & " " & CStr(s)
End Sub
