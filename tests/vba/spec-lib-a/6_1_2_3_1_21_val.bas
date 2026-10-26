' INV: vbal.6.1.2.3.1.21, func.val
' SOURCE: spec ms-vbal 6.1.2.3.1.21 Val: reads numeric prefix, strips blanks/tabs/linefeeds, radix prefixes
' EXPECT: empty=0
' EXPECT: int=12
' EXPECT: dec=3.5
' EXPECT: neg=-5
' EXPECT: text=0
' EXPECT: prefix=12
' EXPECT: blanks=123
' EXPECT: tab=7
' EXPECT: lf=77
' EXPECT: dollar=0
' EXPECT: comma=1
' EXPECT: hex=255
' EXPECT: oct=15
' EXPECT: type=Double
' EXPECT: emptyType=Double
Public Sub Main()
    Debug.Print "empty=" & CStr(Val(""))
    Debug.Print "int=" & CStr(Val("12"))
    Debug.Print "dec=" & CStr(Val("3.5"))
    Debug.Print "neg=" & CStr(Val("-5"))
    Debug.Print "text=" & CStr(Val("abc"))
    Debug.Print "prefix=" & CStr(Val("12abc"))
    Debug.Print "blanks=" & CStr(Val("1 2 3"))
    Debug.Print "tab=" & CStr(Val(vbTab & "7"))
    Debug.Print "lf=" & CStr(Val("7" & vbLf & "7"))
    Debug.Print "dollar=" & CStr(Val("$5"))
    Debug.Print "comma=" & CStr(Val("1,000"))
    Debug.Print "hex=" & CStr(Val("&HFF"))
    Debug.Print "oct=" & CStr(Val("&O17"))
    Debug.Print "type=" & TypeName(Val("1"))
    Debug.Print "emptyType=" & TypeName(Val(""))
End Sub
