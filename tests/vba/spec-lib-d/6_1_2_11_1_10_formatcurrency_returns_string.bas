' INV: vbal.6.1.2.11.1.10, func.formatcurrency
' SOURCE: spec ms-vbal 6.1.2.11.1.10 FormatCurrency returns a String containing the digits; the symbol is implementation-defined
' EXPECT: t=String
' EXPECT: has=True
Public Sub Main()
    Dim s As String
    s = FormatCurrency(1234.5, 2, vbTrue, vbFalse, vbFalse)
    Debug.Print "t=" & TypeName(s)
    Debug.Print "has=" & CStr(InStr(s, "1234.50") > 0)
End Sub
