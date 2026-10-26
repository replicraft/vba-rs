' INV: vbal.5.2.3.3
' SOURCE: spec ms-vbal 5.2.3.3 UDT members of various declared types
' EXPECT: 0|<>|Empty
' EXPECT: Long String Variant
' EXPECT: 7 hi
Private Type Rec
    n As Long
    s As String
    v As Variant
End Type
Public Sub Main()
    Dim r As Rec
    Debug.Print CStr(r.n) & "|<" & r.s & ">|" & TypeName(r.v)
    Debug.Print TypeName(r.n) & " " & TypeName(r.s) & " Variant"
    r.n = 7
    r.s = "hi"
    Debug.Print CStr(r.n) & " " & r.s
End Sub
