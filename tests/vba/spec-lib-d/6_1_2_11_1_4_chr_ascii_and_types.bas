' INV: vbal.6.1.2.11.1.4, func.chr
' SOURCE: spec ms-vbal 6.1.2.11.1.4 Chr 7-bit code points, Chr variant result vs Chr$ String result
' EXPECT: a=A
' EXPECT: b=z
' EXPECT: c=1
' EXPECT: d=0
' EXPECT: e=127
' EXPECT: t1=String
' EXPECT: t2=String
Public Sub Main()
    Debug.Print "a=" & Chr(65)
    Debug.Print "b=" & Chr$(122)
    Debug.Print "c=" & CStr(Len(Chr(0)))
    Debug.Print "d=" & CStr(Asc(Chr(0)))
    Debug.Print "e=" & CStr(Asc(Chr(127)))
    Debug.Print "t1=" & TypeName(Chr(65))
    Debug.Print "t2=" & TypeName(Chr$(65))
End Sub
