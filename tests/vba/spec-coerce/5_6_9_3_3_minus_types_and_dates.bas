' INV: vbal.5.6.9.3.3, op.sub
' SOURCE: spec ms-vbal 5.6.9.3.3 Binary - operator, Date - Date is Double; Date minus a number is Date; Byte-Byte stays Byte
' EXPECT: Double 2.5
' EXPECT: Date 2.5
' EXPECT: Date 4.5
' EXPECT: Integer 2
' EXPECT: Byte 1
' EXPECT: Double -5.5
' EXPECT: Integer -2
Public Sub Main()
    Dim d1 As Date, d2 As Date
    Dim y As Byte, z As Byte
    Dim r As Variant
    d1 = 7: d2 = 4.5
    r = d1 - d2
    Debug.Print TypeName(r) & " " & CStr(r)
    r = d1 - 4.5
    Debug.Print TypeName(r) & " " & CStr(CDbl(r))
    r = 9 - d2
    Debug.Print TypeName(r) & " " & CStr(CDbl(r))
    r = 5 - 3
    Debug.Print TypeName(r) & " " & CStr(r)
    y = 3: z = 2
    r = y - z
    Debug.Print TypeName(r) & " " & CStr(r)
    r = CDbl(1) - 6.5
    Debug.Print TypeName(r) & " " & CStr(r)
    r = y - 5
    Debug.Print TypeName(r) & " " & CStr(r)
End Sub
