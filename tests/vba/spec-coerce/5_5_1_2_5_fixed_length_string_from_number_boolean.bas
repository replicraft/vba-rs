' INV: vbal.5.5.1.2.5, expr.implicit-conv, type.string-fixed
' SOURCE: spec ms-vbal 5.5.1.2.5 Let-coercion to String * length, numeric/Boolean source goes through String first
' EXPECT: [12   ]
' EXPECT: [-1.5 ]
' EXPECT: [True ]
' EXPECT: [False]
' EXPECT: [12345]
' EXPECT: [123]
Public Sub Main()
    Dim f As String * 5
    Dim g As String * 3
    Dim b As Boolean
    f = 12: Debug.Print "[" & f & "]"
    f = -1.5: Debug.Print "[" & f & "]"
    b = True: f = b: Debug.Print "[" & f & "]"
    b = False: f = b: Debug.Print "[" & f & "]"
    f = 1234567: Debug.Print "[" & f & "]"
    g = 12345: Debug.Print "[" & g & "]"
End Sub
