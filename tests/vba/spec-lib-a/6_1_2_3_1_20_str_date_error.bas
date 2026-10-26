' INV: vbal.6.1.2.3.1.20, func.str
' SOURCE: spec ms-vbal 6.1.2.3.1.20 Str of a Date is its String coercion; Str of an Error value is "Error n"
' EXPECT: date=[12:00:00 PM]
Public Sub Main()
    Debug.Print "date=[" & Str(CDate(0.5)) & "]"
End Sub
