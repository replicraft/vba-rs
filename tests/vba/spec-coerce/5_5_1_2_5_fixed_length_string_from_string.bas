' INV: vbal.5.5.1.2.5, expr.implicit-conv, type.string-fixed
' SOURCE: spec ms-vbal 5.5.1.2.5 Let-coercion to String * length, truncation and space padding
' EXPECT: [ab   ]
' EXPECT: [abcde]
' EXPECT: [abcde]
' EXPECT: [     ]
' EXPECT: len=5
Public Sub Main()
    Dim f As String * 5
    f = "ab": Debug.Print "[" & f & "]"
    f = "abcdefgh": Debug.Print "[" & f & "]"
    f = "abcde": Debug.Print "[" & f & "]"
    f = "": Debug.Print "[" & f & "]"
    Debug.Print "len=" & CStr(Len(f))
End Sub
