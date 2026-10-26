' INV: vbal.6.1.2.3.1.1, func.cbool
' SOURCE: spec ms-vbal 6.1.2.3.1.1 CBool value coercion
' EXPECT: zero=False
' EXPECT: five=True
' EXPECT: neg=True
' EXPECT: frac=True
' EXPECT: strTrue=True
' EXPECT: strfalse=False
' EXPECT: strMixed=True
' EXPECT: str0=False
' EXPECT: str2=True
' EXPECT: empty=False
' EXPECT: type=Boolean
Public Sub Main()
    Dim e As Variant
    Debug.Print "zero=" & CStr(CBool(0))
    Debug.Print "five=" & CStr(CBool(5))
    Debug.Print "neg=" & CStr(CBool(-1))
    Debug.Print "frac=" & CStr(CBool(0.4))
    Debug.Print "strTrue=" & CStr(CBool("True"))
    Debug.Print "strfalse=" & CStr(CBool("false"))
    Debug.Print "strMixed=" & CStr(CBool("tRuE"))
    Debug.Print "str0=" & CStr(CBool("0"))
    Debug.Print "str2=" & CStr(CBool("2"))
    Debug.Print "empty=" & CStr(CBool(e))
    Debug.Print "type=" & TypeName(CBool(1))
End Sub
