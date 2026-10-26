' INV: vbal.2.3
' SOURCE: spec ms-vbal 2.3 variables, initial data value of a variable by declared type
' EXPECT: False
' EXPECT: 0 0 0 0 0 0 0
' EXPECT: Double 0
' EXPECT: Single 0
' EXPECT: 0
' EXPECT: [] 0
' EXPECT: Empty
' EXPECT: True
Public Sub Main()
    Dim bo As Boolean
    Dim by As Byte, cu As Currency, db As Double, it As Integer, lg As Long, ll As LongLong, sg As Single
    Dim dt As Date
    Dim s As String
    Dim v As Variant
    Dim o As Object
    Debug.Print CStr(bo)
    Debug.Print CStr(by) & " " & CStr(cu) & " " & CStr(db) & " " & CStr(it) & " " & CStr(lg) & " " & CStr(ll) & " " & CStr(sg)
    Debug.Print TypeName(db) & " " & CStr(db)
    Debug.Print TypeName(sg) & " " & CStr(sg)
    Debug.Print CStr(CDbl(dt))
    Debug.Print "[" & s & "] " & CStr(Len(s))
    Debug.Print TypeName(v)
    Debug.Print CStr(o Is Nothing)
End Sub
