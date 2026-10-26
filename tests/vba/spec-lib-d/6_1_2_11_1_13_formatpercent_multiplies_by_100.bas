' INV: vbal.6.1.2.11.1.13, func.formatpercent
' SOURCE: spec ms-vbal 6.1.2.11.1.13 FormatPercent multiplies by 100 and appends a percent sign
' EXPECT: a=25%
' EXPECT: b=12.50%
' EXPECT: c=100%
' EXPECT: d=(50%)
' EXPECT: t=String
Public Sub Main()
    Debug.Print "a=" & FormatPercent(0.25, 0, vbTrue, vbFalse, vbFalse)
    Debug.Print "b=" & FormatPercent(0.125, 2, vbTrue, vbFalse, vbFalse)
    Debug.Print "c=" & FormatPercent(1, 0, vbTrue, vbFalse, vbFalse)
    Debug.Print "d=" & FormatPercent(-0.5, 0, vbTrue, vbTrue, vbFalse)
    Debug.Print "t=" & TypeName(FormatPercent(0.5))
End Sub
