' INV: vbal.5.5.1.2.4, expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.2.4 Let-coercion to and from String, number to String scientific notation when integral digits exceed the type's maximum
' EXPECT: d1e15=1E+15
' EXPECT: d16 sci=True
' EXPECT: single8 sci=True
' EXPECT: single8 starts=1.23
' EXPECT: neg d16 sci=True
Public Sub Main()
    Dim s As String
    Dim d As Double
    Dim sg As Single
    d = 1E+15: s = d: Debug.Print "d1e15=" & s
    d = 1234567890123456#: s = d
    Debug.Print "d16 sci=" & CStr(InStr(s, "E+") > 0)
    sg = 12345678: s = sg
    Debug.Print "single8 sci=" & CStr(InStr(s, "E+") > 0)
    Debug.Print "single8 starts=" & Left$(s, 4)
    d = -1234567890123456#: s = d
    Debug.Print "neg d16 sci=" & CStr(InStr(s, "E+") > 0 And Left$(s, 1) = "-")
End Sub
