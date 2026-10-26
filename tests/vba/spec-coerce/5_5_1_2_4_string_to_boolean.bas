' INV: vbal.5.5.1.2.4, expr.implicit-conv
' SOURCE: spec ms-vbal 5.5.1.2.4 Let-coercion to and from String, String to Boolean (True/False words, #TRUE#/#FALSE#, otherwise via Double)
' EXPECT: True=True
' EXPECT: False=False
' EXPECT: TRUE=True
' EXPECT: tRuE=True
' EXPECT: fAlSe=False
' EXPECT: #TRUE#=True
' EXPECT: #FALSE#=False
' EXPECT: 1=True
' EXPECT: 0=False
' EXPECT: 0.0=False
' EXPECT: -3=True
' EXPECT: 0.5=True
' EXPECT: 1e-5=True
Public Sub Main()
    Dim b As Boolean
    b = "True": Debug.Print "True=" & CStr(b)
    b = "False": Debug.Print "False=" & CStr(b)
    b = "TRUE": Debug.Print "TRUE=" & CStr(b)
    b = "tRuE": Debug.Print "tRuE=" & CStr(b)
    b = "fAlSe": Debug.Print "fAlSe=" & CStr(b)
    b = "#TRUE#": Debug.Print "#TRUE#=" & CStr(b)
    b = "#FALSE#": Debug.Print "#FALSE#=" & CStr(b)
    b = "1": Debug.Print "1=" & CStr(b)
    b = "0": Debug.Print "0=" & CStr(b)
    b = "0.0": Debug.Print "0.0=" & CStr(b)
    b = "-3": Debug.Print "-3=" & CStr(b)
    b = "0.5": Debug.Print "0.5=" & CStr(b)
    b = "1e-5": Debug.Print "1e-5=" & CStr(b)
End Sub
