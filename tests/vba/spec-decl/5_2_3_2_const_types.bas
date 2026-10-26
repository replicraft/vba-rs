' INV: vbal.5.2.3.2
' SOURCE: spec ms-vbal 5.2.3.2 declared type of constants: suffix, As clause, expression
' EXPECT: Long Integer Double String
' EXPECT: Byte 200
' EXPECT: Currency
Private Const a& = 5
Private Const b = 5
Private Const c = 1.5
Private Const d = "x"
Private Const e As Byte = 200
Private Const f@ = 2
Public Sub Main()
    Debug.Print TypeName(a) & " " & TypeName(b) & " " & TypeName(c) & " " & TypeName(d)
    Debug.Print TypeName(e) & " " & CStr(e)
    Debug.Print TypeName(f)
End Sub
