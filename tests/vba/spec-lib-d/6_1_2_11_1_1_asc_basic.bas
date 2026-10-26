' INV: vbal.6.1.2.11.1.1, func.asc
' SOURCE: spec ms-vbal 6.1.2.11.1.1 Asc returns code of first character, Integer result
' EXPECT: a=65
' EXPECT: b=65
' EXPECT: t=Integer
Public Sub Main()
    Debug.Print "a=" & CStr(Asc("A"))
    Debug.Print "b=" & CStr(Asc("ABC"))
    Debug.Print "t=" & TypeName(Asc("A"))
End Sub
