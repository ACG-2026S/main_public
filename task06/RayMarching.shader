Shader "Custom/RayMarching"
{
    Properties
    {
        _MaxDist ("Max Distance", Float) = 50
        _Epsilon ("Epsilon", Float) = 0.001
        _MaxSteps ("Max Steps", Int) = 128
    }

    SubShader
    {
        Tags { "RenderType"="Opaque" }
        Cull Off

        Pass
        {
            CGPROGRAM
            #pragma vertex vert
            #pragma fragment frag

            #include "UnityCG.cginc"

            float _MaxDist;
            float _Epsilon;
            int _MaxSteps;

            struct appdata
            {
                float4 vertex : POSITION;
            };

            struct v2f
            {
                float4 pos : SV_POSITION;
                float3 worldPos : TEXCOORD0;
            };

            v2f vert(appdata v)
            {
                v2f o;
                o.pos = UnityObjectToClipPos(v.vertex);
                o.worldPos = mul(unity_ObjectToWorld, v.vertex).xyz;
                return o;
            }

            // ----------------------------
            // Signed Distance Function
            // ----------------------------

            float sdCappedCylinder(float3 p, float h, float r)
            {
                float2 d = abs(float2(length(p.xz), p.y)) - float2(r, h);
                return min(max(d.x, d.y), 0.0) + length(max(d, 0.0));
            }

            static const float len_cylinder = 0.8;
            static const float rad_cylinder = 0.45;
            static const float rad_sphere = 0.8;
            static const float box_size = 0.6;

            float sceneSDF(float3 pos)
            {
                float dCyl1 = sdCappedCylinder(pos.xyz, len_cylinder, rad_cylinder);
                return dCyl1;
            }            

            float3 SDF_color(float3 pos)
            {
                return float3(0.0, 1.0, 0.0);
            }

            // Normal estimation with finite differences
            float3 estimateNormal(float3 p)
            {
                float e = 0.001;

                float dx = sceneSDF(p + float3(e,0,0)) - sceneSDF(p - float3(e,0,0));
                float dy = sceneSDF(p + float3(0,e,0)) - sceneSDF(p - float3(0,e,0));
                float dz = sceneSDF(p + float3(0,0,e)) - sceneSDF(p - float3(0,0,e));

                return normalize(float3(dx, dy, dz));
            }

            // Ray marching
            bool rayMarch(float3 ro, float3 rd, out float t)
            {
                t = 0.0;

                for (int i = 0; i < _MaxSteps; i++)
                {
                    float3 p = ro + t * rd;
                    float d = sceneSDF(p);

                    if (d < _Epsilon)
                    {
                        return true;
                    }

                    t += d;

                    if (t > _MaxDist)
                    {
                        break;
                    }
                }

                return false;
            }

            fixed4 frag(v2f i) : SV_Target
            {
                // camera position
                float3 ro = _WorldSpaceCameraPos;

                // the ray direction from camera to pixel
                float3 rd = normalize(i.worldPos - ro);

                float t;
                bool hit = rayMarch(ro, rd, t);

                if (!hit)
                {
                    discard;
                    return fixed4(0,0,0,0);
                }

                float3 p = ro + t * rd;
                float3 n = estimateNormal(p);

                // simple lighting
                float3 lightDir = normalize(float3(0.5, 1.0, 0.8));
                float diff = max(0.0, dot(n, lightDir));

                float3 col = SDF_color(p) * (0.2 + 0.8 * diff);

                return fixed4(col, 1.0);
            }

            ENDCG
        }
    }
}