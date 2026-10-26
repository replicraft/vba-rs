' INV: vbal.6.1.2.3.1.5, vbal.5.5.1.2.4, func.cdbl
' SOURCE: spec ms-vbal 6.1.2.3.1.5 CDbl numeric-coercion-string allows whitespace between sign and digits
' XFAIL: wrong not implemented: whitespace between a sign and the digits of a numeric string
' EXPECT: signws=-3
' EXPECT: plusws=3
' EXPECT: expSign=0.015
Public Sub Main()
    Debug.Print "signws=" & CStr(CDbl("- 3"))
    Debug.Print "plusws=" & CStr(CDbl(" + 3 "))
    Debug.Print "expSign=" & CStr(CDbl("1.5e-2"))
End Sub
