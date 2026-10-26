' INV: vbal.5.2.3.2
' SOURCE: spec ms-vbal 5.2.3.2 constant binding is the value Let-coerced to declared type
' EXPECT: Long 12
' EXPECT: String 5
' EXPECT: Double 3
Private Const a As Long = "12"
Private Const b As String = 5
Private Const c As Double = 3
Public Sub Main()
    Debug.Print TypeName(a) & " " & CStr(a)
    Debug.Print TypeName(b) & " " & CStr(b)
    Debug.Print TypeName(c) & " " & CStr(c)
End Sub
