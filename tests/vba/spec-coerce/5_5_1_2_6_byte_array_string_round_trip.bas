' INV: vbal.5.5.1.2.6, expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.2.6 Let-coercion to and from resizable Byte(), String to Byte() and back round-trips; uninitialized array gives empty string
' EXPECT: back=Hello
' EXPECT: lenok=True
' EXPECT: fixed=[Hi   ]
' EXPECT: empty len=0
' EXPECT: modified=Jello
Public Sub Main()
    Dim b() As Byte
    Dim s As String
    Dim f As String * 5
    Dim e() As Byte
    b = "Hello"
    s = b
    Debug.Print "back=" & s
    Debug.Print "lenok=" & CStr(UBound(b) >= 4)
    b = "Hi"
    f = b
    Debug.Print "fixed=[" & f & "]"
    s = e
    Debug.Print "empty len=" & CStr(Len(s))
    b = "Hello"
    b(0) = 74
    s = b
    Debug.Print "modified=" & s
End Sub
