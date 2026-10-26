' INV: vbal.3.2.2, lex.continuation
' SOURCE: spec ms-vbal 3.2.2 logical line grammar, line continuation after whitespace (space or tab) joins physical lines
' EXPECT: x=3
' EXPECT: x=7
' EXPECT: x=18
Public Sub Main()
    Dim x As Long
    x = 1 + _
        2
    Debug.Print "x=" & CStr(x)
    x = x +	_
        4
    Debug.Print "x=" & CStr(x)
    x = x + _
        5 + _
        6
    Debug.Print "x=" & CStr(x)
End Sub
