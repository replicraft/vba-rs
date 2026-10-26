' INV: vbal.6.1.2.11.1.12, func.formatnumber
' SOURCE: spec ms-vbal 6.1.2.11.1.12 FormatNumber digits after decimal, leading digit, parentheses
' EXPECT: a=1234.50
' EXPECT: b=0.5
' EXPECT: c=.5
' EXPECT: d=(5)
' EXPECT: e=-5
' EXPECT: t=String
Public Sub Main()
    Debug.Print "a=" & FormatNumber(1234.5, 2, vbTrue, vbFalse, vbFalse)
    Debug.Print "b=" & FormatNumber(0.5, 1, vbTrue, vbFalse, vbFalse)
    Debug.Print "c=" & FormatNumber(0.5, 1, vbFalse, vbFalse, vbFalse)
    Debug.Print "d=" & FormatNumber(-5, 0, vbTrue, vbTrue, vbFalse)
    Debug.Print "e=" & FormatNumber(-5, 0, vbTrue, vbFalse, vbFalse)
    Debug.Print "t=" & TypeName(FormatNumber(1))
End Sub
