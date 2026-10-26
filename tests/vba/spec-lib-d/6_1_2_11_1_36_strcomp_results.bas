' INV: vbal.6.1.2.11.1.36, func.strcomp
' SOURCE: spec ms-vbal 6.1.2.11.1.36 StrComp returns -1, 0, 1 or Null; Compare argument
' EXPECT: lt=-1
' EXPECT: eq=0
' EXPECT: gt=1
' EXPECT: n1=True
' EXPECT: n2=True
' EXPECT: bin=1
' EXPECT: txt=0
' EXPECT: t=Integer
' EXPECT: pre=-1
Public Sub Main()
    Debug.Print "lt=" & CStr(StrComp("a", "b"))
    Debug.Print "eq=" & CStr(StrComp("abc", "abc"))
    Debug.Print "gt=" & CStr(StrComp("b", "a"))
    Debug.Print "n1=" & CStr(IsNull(StrComp(Null, "a")))
    Debug.Print "n2=" & CStr(IsNull(StrComp("a", Null)))
    Debug.Print "bin=" & CStr(StrComp("a", "A", vbBinaryCompare))
    Debug.Print "txt=" & CStr(StrComp("a", "A", vbTextCompare))
    Debug.Print "t=" & TypeName(StrComp("a", "b"))
    Debug.Print "pre=" & CStr(StrComp("ab", "abc"))
End Sub
